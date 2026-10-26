' INV: op.like
' SOURCE: derived brackets match special characters literally (like-operator page)
' EXPECT: True
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr("a?c" Like "a[?]c")
    Debug.Print CStr("a#c" Like "a[#]c")
    Debug.Print CStr("abc" Like "a[?]c")
End Sub
