/* ------------------------------------------------------------
author: "Grame"
copyright: "(c)GRAME 2006"
license: "BSD"
name: "echo"
version: "1.0"
Code generated with Faust 2.0.a41 (http://faust.grame.fr)
------------------------------------------------------------ */
#ifndef FAUSTFLOAT
#define FAUSTFLOAT float
#endif  

#include <math.h>


#ifndef FAUSTCLASS 
#define FAUSTCLASS mydsp
#endif

class mydsp : public dsp {
	
  private:
	
	float fRec0[131072];
	FAUSTFLOAT fHslider0;
	int fSamplingFreq;
	float fConst0;
	FAUSTFLOAT fHslider1;
	int IOTA;
	
  public:
	
	void static metadata(Meta* m) { 
		m->declare("author", "Grame");
		m->declare("copyright", "(c)GRAME 2006");
		m->declare("license", "BSD");
		m->declare("math.lib/author", "GRAME");
		m->declare("math.lib/copyright", "GRAME");
		m->declare("math.lib/license", "LGPL with exception");
		m->declare("math.lib/name", "Math Library");
		m->declare("math.lib/version", "1.0");
		m->declare("music.lib/author", "GRAME");
		m->declare("music.lib/copyright", "GRAME");
		m->declare("music.lib/license", "LGPL with exception");
		m->declare("music.lib/name", "Music Library");
		m->declare("music.lib/version", "1.0");
		m->declare("name", "echo");
		m->declare("version", "1.0");
	}

	virtual int getNumInputs() {
		return 1;
		
	}
	virtual int getNumOutputs() {
		return 1;
		
	}
	virtual int getInputRate(int channel) {
		int rate;
		switch (channel) {
			case 0: {
				rate = 1;
				break;
			}
			default: {
				rate = -1;
				break;
			}
			
		}
		return rate;
		
	}
	virtual int getOutputRate(int channel) {
		int rate;
		switch (channel) {
			case 0: {
				rate = 1;
				break;
			}
			default: {
				rate = -1;
				break;
			}
			
		}
		return rate;
		
	}
	
	static void classInit(int samplingFreq) {
		
	}
	
	virtual void instanceInit(int samplingFreq) {
		fSamplingFreq = samplingFreq;
		fHslider0 = FAUSTFLOAT(0.);
		fConst0 = (0.001f * float(min(192000, max(1, fSamplingFreq))));
		fHslider1 = FAUSTFLOAT(0.);
		IOTA = 0;
		for (int i0 = 0; (i0 < 131072); i0 = (i0 + 1)) {
			fRec0[i0] = 0.f;
			
		}
		
	}
	
	virtual void init(int samplingFreq) {
		classInit(samplingFreq);
		instanceInit(samplingFreq);
	}
	
	virtual void buildUserInterface(UI* interface) {
		interface->openVerticalBox("echo-simple");
		interface->openVerticalBox("echo  1000");
		interface->addHorizontalSlider("feedback", &fHslider0, 0.f, 0.f, 100.f, 0.1f);
		interface->addHorizontalSlider("millisecond", &fHslider1, 0.f, 0.f, 1000.f, 0.1f);
		interface->closeBox();
		interface->closeBox();
		
	}
	
	virtual void compute(int count, FAUSTFLOAT** inputs, FAUSTFLOAT** outputs) {
		FAUSTFLOAT* input0 = inputs[0];
		FAUSTFLOAT* output0 = outputs[0];
		float fSlow0 = (0.01f * float(fHslider0));
		int iSlow1 = (1 + ((int((fConst0 * float(fHslider1))) - 1) & 65535));
		for (int i = 0; (i < count); i = (i + 1)) {
			fRec0[(IOTA & 131071)] = ((fSlow0 * fRec0[((IOTA - iSlow1) & 131071)]) + float(input0[i]));
			output0[i] = FAUSTFLOAT(fRec0[((IOTA - 0) & 131071)]);
			IOTA = (IOTA + 1);
			
		}
		
	}

	
};

