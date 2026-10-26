' INV: func.space
' SOURCE: spec ms-vbal 6.1.2.11.1.33 Space
' EXPECT: 5
' EXPECT: []
Public Sub Main()
    Debug.Print CStr(Len(Space(5)))
    Debug.Print CStr("[" & Space(0) & "]")
End Sub
