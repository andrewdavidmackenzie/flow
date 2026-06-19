# FFT — Fast Fourier Transform

This example computes the Fast Fourier Transform of a signal consisting
of two sine waves (440 Hz and 880 Hz) sampled at 8000 Hz.

The flow generates sample data, passes it through an FFT function
(provided as a custom WASM implementation), and outputs the dominant
frequencies found in the signal.
