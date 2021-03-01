""" Check for missing runtime binaries. """

import shutil


def check_bin_dependency(bins):
    """ Check for missing runtime binaries. """
    for bin in bins:
        if shutil.which(bin) is None:
            raise RuntimeError("Binary '%s' could not be found" % (bin))
