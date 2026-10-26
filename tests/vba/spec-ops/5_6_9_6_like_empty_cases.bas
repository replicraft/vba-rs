' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like both strings fully matched or one exhausted
' EXPECT: True
' EXPECT: True
' EXPECT: False
' EXPECT: False
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr("" Like "")
    Debug.Print CStr("" Like "*")
    Debug.Print CStr("" Like "?")
    Debug.Print CStr("a" Like "")
    Debug.Print CStr("ab" Like "a")
    Debug.Print CStr("a" Like "a*")
End Sub
