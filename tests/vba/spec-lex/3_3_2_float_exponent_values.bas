' INV: vbal.3.3.2, lit.float
' SOURCE: spec ms-vbal 3.3.2 FLOAT tokens, value of exponent forms, D and E letters, leading dot
' EXPECT: 1000
' EXPECT: 1000
' EXPECT: 0.25
' EXPECT: 0.5
' EXPECT: 150
' EXPECT: True
' EXPECT: 150
' EXPECT: 5
Public Sub Main()
    Debug.Print CStr(1E3)
    Debug.Print CStr(1d3)
    Debug.Print CStr(2.5E-1)
    Debug.Print CStr(.5)
    Debug.Print CStr(1.5E+2)
    Debug.Print CStr(12E-1 = 1.2)
    Debug.Print CStr(1.5D2)
    Debug.Print CStr(5.0)
End Sub
