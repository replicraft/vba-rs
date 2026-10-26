' INV: op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like operator
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr("" Like "*")
    Debug.Print CStr("" Like "?")
    Debug.Print CStr("abc" Like "a?c")
    Debug.Print CStr("abc" Like "a?d")
End Sub
