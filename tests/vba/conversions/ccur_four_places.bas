' INV: func.ccur
' SOURCE: derived Currency keeps four decimal places
' EXPECT: 1.2346
' EXPECT: 2
Public Sub Main()
    Debug.Print CStr(CCur(1.23456))
    Debug.Print CStr(CCur(2))
End Sub
