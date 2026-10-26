' INV: vbal.5.6.9.5, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational String under Option Compare Text is case-insensitive
' EXPECT: True
' EXPECT: True
' EXPECT: False
' EXPECT: True
Option Compare Text
Public Sub Main()
    Debug.Print CStr("abc" = "ABC")
    Debug.Print CStr("ABC" >= "abc")
    Debug.Print CStr("abc" <> "ABC")
    Debug.Print CStr("abc" < "abd")
End Sub
