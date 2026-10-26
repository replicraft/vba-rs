' INV: op.eq
' SOURCE: spec ms-vbal 5.2.1.1 Option Compare default binary
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr("a" = "A")
    Debug.Print CStr("abc" = "abc")
End Sub
