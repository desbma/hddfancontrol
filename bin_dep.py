import shutil


def check_bin_dependency(bins):
  for bin in bins:
    if shutil.which(bin) is None:
      raise RuntimeError("Binary '%s' could not be found" % (bin))
