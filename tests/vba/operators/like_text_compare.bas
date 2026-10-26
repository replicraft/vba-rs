' INV: op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like operator text-compare mode
' EXPECT: True
' EXPECT: True
Option Compare Text
Public Sub Main()
    Debug.Print CStr("ABC" Like "abc")
    Debug.Print CStr("e" Like "[A-E]")
End Sub
