' INV: vbal.6.1.2.10.1.10, func.sqr
' SOURCE: spec ms-vbal 6.1.2.10.1.10 Sqr square root as Double
' EXPECT: 2
' EXPECT: 12
' EXPECT: 0
' EXPECT: Double
Public Sub Main()
    Debug.Print CStr(Sqr(4))
    Debug.Print CStr(Sqr(144))
    Debug.Print CStr(Sqr(0))
    Debug.Print TypeName(Sqr(9))
End Sub
