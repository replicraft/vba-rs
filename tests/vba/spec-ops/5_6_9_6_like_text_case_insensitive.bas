' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like under Option Compare Text is case insensitive
' EXPECT: True
' EXPECT: True
' EXPECT: True
Option Compare Text
Public Sub Main()
    Debug.Print CStr("ABC" Like "abc")
    Debug.Print CStr("B" Like "[a-c]")
    Debug.Print CStr("abc" Like "A*C")
End Sub
