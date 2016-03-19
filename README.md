# fraust-echo
run the faust echo example from rust, using rust-portaudio to interface with the audio hardware.

Just compile the project normally with cargo build and run with ./target/debug/fraust-echo

cpp/basic.cpp is the shim code between the cpp code faust generates and C functions that rust can use.  

If you want to tweak or replace the faust algorithm, use faust to generate a new cpp file and replace 'echo.cpp' with your new filename in basic.cpp.  

To control the echo parameters you can use oscpad (its on cargo).  Install it and call 

$> oscpad echoconfig.json

from the fraust-echo directory.  Then go to localhost:3030 with your web browser.  

