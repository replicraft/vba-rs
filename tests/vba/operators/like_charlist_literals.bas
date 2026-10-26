' INV: op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like operator
' EXPECT: True
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr("a?c" Like "a[?]c")
    Debug.Print CStr("a#c" Like "a[#]c")
    Debug.Print CStr("abc" Like "a[?]c")
End Sub
