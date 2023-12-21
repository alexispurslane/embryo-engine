import math

samples = []

with open("samples.log", "r") as f:
    for sample in f:
        if len(sample.strip()) > 0:
            samples.append([int(x) for x in sample.strip().split(" ")])

for histogram in samples:
    print("------------------------------------------------------------------\n")
    print("SAMPLE 1")
    print("------------------------------------------------------------------\n")
    pixels = sum(histogram)

    width = 65
    print("Histogram: ")
    for i in range(0, 255):
        ratio = histogram[i] / pixels
        print(histogram[i], end="\t|")
        print(":" * math.ceil(ratio * 1000))

    print(f"Total pixels: {pixels}")

    histogramShared = [x * i for i, x in enumerate(histogram)]
    print(" ".join([str(x) for x in histogramShared]))
    for i in range(1, int(math.ceil(math.log2(len(histogram)))) + 1):
        cutoff = int(math.ceil(len(histogram) / 2**i))
        for j in range(0, cutoff):
            histogramShared[j] += histogramShared[j + cutoff]

    print("")
    print(f"Weighted sum: {histogramShared[0]}")

    wla = histogramShared[0] / max(1920 * 1080, 1.0) - 1.0
    print(f"Weighted log average: {wla}")

    minLogLum = -8.0
    maxLogLum = 3.5
    inverseLogLumRange = 1.0 / (maxLogLum - minLogLum)
    llr = maxLogLum - minLogLum
    wal = 2 ** (((wla / 254.0) * llr) + minLogLum)
    print(f"Weighted average: {wal}")
    print("------------------------------------------------------------------\n")


def colorToBin(r, g, b):
    lum = r * 0.21 + g * 0.71 + b * -0.07
    print(math.log2(lum) - minLogLum)
    if lum < 0.005:
        return 0

    logLum = min(max((math.log2(lum) - minLogLum) * inverseLogLumRange, 0.0), 1.0)

    return int(logLum * 254.0 + 1.0)


print(colorToBin(0.1, 0.1, 0.1))
print(colorToBin(20, 20, 20))
print(colorToBin(20, 0.1, 30))
