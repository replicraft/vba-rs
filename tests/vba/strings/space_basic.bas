' INV: func.space
' SOURCE: derived Space(n) is n blanks
' EXPECT: 5
' EXPECT: []
Public Sub Main()
    Debug.Print CStr(Len(Space(5)))
    Debug.Print CStr("[" & Space(0) & "]")
End Sub
