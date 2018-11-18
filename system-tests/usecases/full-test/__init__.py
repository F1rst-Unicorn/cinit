import os
import unittest
from driver import *

class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace()                                              \
                .that(Sequential(                                           \
                        ChildSpawned("first"),                              \
                        ChildExited("first")))                              \
                .then(Parallel(                                             \
                        Sequential(                                         \
                                ChildSpawned("second"),                     \
                                ChildExited("second")),                     \
                        Sequential(                                         \
                                ChildSpawned("third"),                      \
                                ChildExited("third"))))                     \
                .then(Sequential(                                           \
                        ChildSpawned("fourth"),                             \
                        ChildExited("fourth")))                             \


