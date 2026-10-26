' INV: op.eq
' SOURCE: spec ms-vbal 5.2.1.1 Option Compare Text
' EXPECT: True
' EXPECT: False
Option Compare Text
Public Sub Main()
    Debug.Print CStr("a" = "A")
    Debug.Print CStr("abc" = "ABD")
End Sub
