' INV: func.ccur
' SOURCE: spec ms-vbal 6.1.2.3.1.3 CCur
' EXPECT: 1.2346
' EXPECT: 2
Public Sub Main()
    Debug.Print CStr(CCur(1.23456))
    Debug.Print CStr(CCur(2))
End Sub
