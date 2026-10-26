' INV: func.csng
' SOURCE: derived returns a Single
' EXPECT: Single
' EXPECT: 0.5
Public Sub Main()
    Debug.Print CStr(TypeName(CSng(1)))
    Debug.Print CStr(CSng(0.5))
End Sub
