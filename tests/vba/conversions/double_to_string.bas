' INV: func.cstr, type.double
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 1.4142135623731
' EXPECT: 0.333333333333333
' EXPECT: 0.666666666666667
' EXPECT: 33.3333333333333
' EXPECT: 0.3
' EXPECT: 123456789.123457
' EXPECT: 1E+22
' EXPECT: 1E-10
' EXPECT: -0.5
' EXPECT: 9.00719925474099E+15
' EXPECT: 1E+15
Public Sub Main()
    Debug.Print CStr(Sqr(2))
    Debug.Print CStr(1 / 3)
    Debug.Print CStr(2 / 3)
    Debug.Print CStr(100 / 3)
    Debug.Print CStr(0.1 + 0.2)
    Debug.Print CStr(123456789.123456789)
    Debug.Print CStr(1E22)
    Debug.Print CStr(1E-10)
    Debug.Print CStr(-0.5)
    Debug.Print CStr(2 ^ 53)
    Debug.Print CStr(1E15 + 0.5)
End Sub
