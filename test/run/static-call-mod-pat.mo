func go () {
  let { foobar1 } = module { public func foobar1() = () };
  foobar1();
};
go();

let { foobar2 } = module { public func foobar2() = () };
foobar2();

// CHECK-LABEL: func $init
// CHECK-NOT: call_indirect
// CHECK: call $foobar2

// CHECK-LABEL: func $go
// CHECK-NOT: call_indirect
// CHECK: call $foobar1
