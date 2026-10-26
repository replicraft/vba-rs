' INV: op.eq
' SOURCE: derived Option Compare Binary is the default (like-operator page)
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr("a" = "A")
    Debug.Print CStr("abc" = "abc")
End Sub
