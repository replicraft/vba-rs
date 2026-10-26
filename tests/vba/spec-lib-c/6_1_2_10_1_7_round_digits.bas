' INV: vbal.6.1.2.10.1.7, func.round
' SOURCE: spec ms-vbal 6.1.2.10.1.7 Round to a number of decimal places; integers when omitted
' EXPECT: 2.57
' EXPECT: 5.556
' EXPECT: 3
' EXPECT: 4
' EXPECT: -2.57
Public Sub Main()
    Debug.Print CStr(Round(2.567, 2))
    Debug.Print CStr(Round(5.5555, 3))
    Debug.Print CStr(Round(3.2))
    Debug.Print CStr(Round(3.9))
    Debug.Print CStr(Round(-2.567, 2))
End Sub
