import os
import unittest
from test_driver import *

class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace()                                              \
                .that(CycleDetected(self))


