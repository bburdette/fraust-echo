/* ------------------------------------------------------------
author: "Grame"
copyright: "(c)GRAME 2009"
license: "BSD"
name: "Noise"
version: "1.1"
Code generated with Faust 2.0.a41 (http://faust.grame.fr)
------------------------------------------------------------ */
#ifndef FAUSTFLOAT
#define FAUSTFLOAT float
#endif  



#ifndef FAUSTCLASS 
#define FAUSTCLASS mydsp
#endif

class mydsp : public dsp {
	
  private:
	
	int iRec0[2];
	FAUSTFLOAT fVslider0;
	int fSamplingFreq;
	
  public:
	
	void static metadata(Meta* m) { 
		m->declare("author", "Grame");
		m->declare("copyright", "(c)GRAME 2009");
		m->declare("license", "BSD");
		m->declare("name", "Noise");
		m->declare("version", "1.1");
	}

	virtual int getNumInputs() {
		return 0;
		
	}
	virtual int getNumOutputs() {
		return 1;
		
	}
	virtual int getInputRate(int channel) {
		int rate;
		switch (channel) {
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
		fVslider0 = FAUSTFLOAT(0.5);
		for (int i0 = 0; (i0 < 2); i0 = (i0 + 1)) {
			iRec0[i0] = 0;
			
		}
		
	}
	
	virtual void init(int samplingFreq) {
		classInit(samplingFreq);
		instanceInit(samplingFreq);
	}
	
	virtual void buildUserInterface(UI* interface) {
		interface->openVerticalBox("0x00");
		interface->declare(&fVslider0, "style", "knob");
		interface->addVerticalSlider("Volume", &fVslider0, 0.5f, 0.f, 1.f, 0.1f);
		interface->closeBox();
		
	}
	
	virtual void compute(int count, FAUSTFLOAT** inputs, FAUSTFLOAT** outputs) {
		FAUSTFLOAT* output0 = outputs[0];
		float fSlow0 = (4.65661e-10f * float(fVslider0));
		for (int i = 0; (i < count); i = (i + 1)) {
			iRec0[0] = (12345 + (1103515245 * iRec0[1]));
			output0[i] = FAUSTFLOAT((fSlow0 * float(iRec0[0])));
			iRec0[1] = iRec0[0];
			
		}
		
	}

	
};

