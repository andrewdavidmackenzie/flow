## contrast_stretch

Apply contrast stretching to a flat array of pixel values. Maps the input
range [min, max] to the full output range [0, 255].

Formula: `new_pixel = (pixel - min) * 255 / (max - min)`
