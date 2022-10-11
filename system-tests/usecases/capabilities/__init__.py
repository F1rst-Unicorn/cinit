#  cinit: process initialisation program for containers
#  Copyright (C) 2019 The cinit developers
#
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU General Public License as published by
#  the Free Software Foundation, either version 3 of the License, or
#  (at your option) any later version.
#
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#  GNU General Public License for more details.
#
#  You should have received a copy of the GNU General Public License
#  along with this program.  If not, see <https://www.gnu.org/licenses/>.

from driver import *


class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace()                                              \
                .that(Sequential(
                        Parallel(
                            Sequential(
                                ChildSpawned("some-caps"),
                                ChildExited("some-caps")
                            ),
                            Sequential(
                                ChildSpawned("no-caps"),
                                ChildExited("no-caps")
                            )
                        ),
                        Exited()))                              \

        self.assert_exit_code(0)

        ChildProcess("some-caps", self) \
            .assert_arg("-o") \
            .assert_arg("system-tests/child-dump/some-caps.yml") \
            .assert_uid(1409) \
            .assert_gid(1409) \
            .assert_default_env() \
            .assert_capabilities({"CAP_NET_RAW": 'epia',
                                  "CAP_KILL": 'epia'})

        ChildProcess("no-caps", self) \
            .assert_arg("-o") \
            .assert_arg("system-tests/child-dump/no-caps.yml") \
            .assert_uid(1409) \
            .assert_gid(1409) \
            .assert_default_env() \
            .assert_capabilities({})

        ChildProcess("root-gets-all-caps", self) \
            .assert_arg("-o") \
            .assert_arg("system-tests/child-dump/root-gets-all-caps.yml") \
            .assert_uid(0) \
            .assert_gid(0) \
            .assert_default_env() \
            .assert_capabilities_at_least({
                'CAP_NET_RAW': 'epia',
                'CAP_KILL': 'epia',
                'CAP_CHOWN': 'ep',
                'CAP_DAC_OVERRIDE': 'ep',
                'CAP_FOWNER': 'ep',
                'CAP_FSETID': 'ep',
                'CAP_SETGID': 'ep',
                'CAP_SETUID': 'ep',
                'CAP_SETPCAP': 'ep',
                'CAP_NET_BIND_SERVICE': 'ep',
                'CAP_MKNOD': 'ep',
                'CAP_AUDIT_WRITE': 'ep',
                'CAP_SETFCAP': 'ep',
                'CAP_SYS_CHROOT': 'ep',
                'CAP_SETUID': 'ep',
            })
