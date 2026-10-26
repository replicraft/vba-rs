' INV: op.like
' SOURCE: derived ? matches one character, * zero or more (like-operator page)
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
