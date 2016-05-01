extern crate libc;

use std::ffi::CString;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::io::{Error,ErrorKind};
use std::string::String;
use std::fmt::format;
use std::sync::mpsc;
use std::thread;
use std::str::FromStr;
use std::cmp::min;

extern crate portaudio;

// use portaudio as pa;
use portaudio::{stream, hostapi, device};

extern crate tinyosc;
use tinyosc as osc;

extern {
  pub fn fraust_init(samplerate: i32);
  pub fn fraust_compute(count: i32, input: *const libc::c_float, output: *mut libc::c_float );
  pub fn fraust_setval(label: *const libc::c_char , val: libc::c_float); 
}


enum SeType { 
  SliderPress,
  SliderMove,
  SliderUnpress,
} 

enum SeWhat { 
  Millisecond,
  Feedback
}

pub struct SliderEvt { 
  evttype: SeType
, what: SeWhat
, position: f32
}

const CHANNELS: i32 = 2;
const NUM_SECONDS: i32 = 5;
const SAMPLE_RATE: f64 = 44100.0;
// const FRAMES_PER_BUFFER: u32 = 64;
const FRAMES_PER_BUFFER: u32 = 2048;

fn main()
{
    portaudio::initialize().unwrap();
    print_devs();
    // println!("{:?}", demo());
    callback_demo();
    portaudio::terminate().unwrap();
}

fn print_devs()
{
    for i in 0 .. portaudio::device::get_count().unwrap()
    {
        match portaudio::device::get_info(i)
        {
            None => {},
            Some(info) => println!("{}: {}", i, info.name),
        }
    }
}

fn callback_demo()
{
    // ---------------------------------------------
    // start the osc receiver thread
    // ---------------------------------------------

    // make a channel to receive updates from the osc.
    let (tx, rx) = mpsc::channel::<SliderEvt>();

    // we'll do osc receive below, in the main thread.

    // ---------------------------------------------
    // init fraust 
    // ---------------------------------------------
    println!("initting with sample rate: {}", SAMPLE_RATE);

    unsafe { fraust_init(SAMPLE_RATE as i32); }

    let bufmax = 10000;
    let mut inflts = [0.0;10000];
    inflts[0] = 1.0;

    let mut outflts = [0.0;10000];

    let volstring = CString::new("Volume").unwrap();
    let millisecond = CString::new("millisecond").unwrap();
    let feedback = CString::new("feedback").unwrap();

    unsafe { fraust_setval(feedback.as_ptr(), 50.0); }
    unsafe { fraust_setval(millisecond.as_ptr(), 70.0); }
    // unsafe { fraust_setval(volstring.as_ptr(), 0.05); }

    let mut loopcount = 0;
    let mut buflen = 0;
    let bufmaxu = bufmax as usize;
    let mut bufidx = bufmaxu - 1;

    // ---------------------------------------------
    // set up portaudio callback ftn.
    // ---------------------------------------------
    let callback = Box::new(|input: &[f32], output: &mut [f32], _time: stream::StreamTimeInfo, _flags: stream::StreamCallbackFlags| -> stream::StreamCallbackResult
    {
        // println!("in the callback! frames: {}", frames);
        // any events to update the DSP with?? 
        match rx.try_recv() { 
          Ok(se) => {
            match se.what { 
              SeWhat::Millisecond => { 
                  // println!("setting vol to 0.3!");
                   unsafe { fraust_setval(millisecond.as_ptr(), se.position); }
                }
              SeWhat::Feedback => { 
                  // println!("setting vol to 0.001!");
                  unsafe { fraust_setval(feedback.as_ptr(), se.position); }
                }
            }
          }
          _ => {}
        }

        // do dsp!

        // TO DO: verify input buflen too, in case it has fewer or no channels, thats a segfault.

        unsafe { fraust_compute(output.len() as i32, input.as_ptr(), output.as_mut_ptr()); }

        stream::StreamCallbackResult::Continue
    });

    // ---------------------------------------------
    // start portaudio 
    // ---------------------------------------------
    let finished_callback = Box::new(|| println!("Finshed callback called"));
    let mut stream = match stream::Stream::open_default(2, 2, 44100f64, stream::FRAMES_PER_BUFFER_UNSPECIFIED, Some(callback))
    {
        Err(v) => { println!("Err({:?})", v); return },
        Ok(stream) => stream,
    };


    println!("finished_callback: {:?}", stream.set_finished_callback(finished_callback));
    println!("start: {:?}", stream.start());

    /*
    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("stop: {:?}", stream.stop());

    println!("finished_callback: {:?}", stream.unset_finished_callback());
    println!("start: {:?}", stream.start());
    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("stop: {:?}", stream.stop());
    */

    let oscrecvip = std::net::SocketAddr::from_str("0.0.0.0:8000").expect("Invalid IP");
    // spawn the osc receiver thread. 
    match oscthread(oscrecvip, tx) {
      Ok(s) => println!("oscthread exited ok"),
      Err(e) => println!("oscthread error: {} ", e),
    };
}

fn oscthread(oscrecvip: SocketAddr, sender: mpsc::Sender<SliderEvt>) -> Result<String, Error> { 
  let socket = try!(UdpSocket::bind(oscrecvip));
  let mut buf = [0; 1000];

  loop { 
    let (amt, src) = try!(socket.recv_from(&mut buf));

    // println!("length: {}", amt);
    let inmsg = match osc::Message::deserialize(&buf[.. amt]) {
      Ok(m) => m,
      Err(e) => {
          return Err(Error::new(ErrorKind::Other, "oh no!"));
        },
      };

    // println!("message received {} {:?}", inmsg.path, inmsg.arguments );
    match inmsg {
      osc::Message { path: ref path, arguments: ref args } => {
        if args.len() > 1 {
          match (&args[0], &args[1]) {
            (&osc::Argument::s(etype), &osc::Argument::f(pos)) => {
            let what = match path { 
              &"millisecond" => Some(SeWhat::Millisecond),
              &"feedback" => Some(SeWhat::Feedback),
              _ => None,
              };

            let setype = match etype { 
              "s_pressed" => Some(SeType::SliderPress),
              "s_unpressed" => Some(SeType::SliderUnpress),
              "s_moved" => Some(SeType::SliderMove),
              _ => None,
              };

            match (what, setype) { 
              (Some(what), Some(sevt)) => { 
                let amt = match what { 
                  SeWhat::Millisecond => pos * 500.0,
                  SeWhat::Feedback => pos * 100.0,
                  };
                
                let se = SliderEvt{ evttype: sevt, what: what, position: amt };
                sender.send(se)
              }
              _ => Ok(())
            }
            },
            _ => Ok(())
          } 
        }
        else {
          Ok(())
        }
      },
      };
    };

  // drop(socket); // close the socket
  // Ok(String::from("meh"))
}


