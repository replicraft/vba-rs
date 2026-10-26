' INV: func.len
' SOURCE: derived Len returns the number of characters
' EXPECT: 3
' EXPECT: 0
' EXPECT: 5
' EXPECT: 3
Public Sub Main()
    Debug.Print CStr(Len("abc"))
    Debug.Print CStr(Len(""))
    Debug.Print CStr(Len(12345))
    Debug.Print CStr(Len("a b"))
End Sub
