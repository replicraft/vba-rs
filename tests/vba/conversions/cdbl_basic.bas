' INV: func.cdbl
' SOURCE: spec ms-vbal 6.1.2.3.1.5 CDbl
' EXPECT: 1.5
' EXPECT: 1.25
' EXPECT: Double
Public Sub Main()
    Debug.Print CStr(CDbl("1.5"))
    Debug.Print CStr(CDbl(CCur(1.25)))
    Debug.Print CStr(TypeName(CDbl(1)))
End Sub
