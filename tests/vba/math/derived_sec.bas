' INV: derived.sec, func.cos
' SOURCE: spec ms-vbal 6.1.2.10.1.3 Cos
' EXPECT: 1
' EXPECT: 2
Public Sub Main()
    Debug.Print CStr(Round(1 / Cos(0), 10))
    Debug.Print CStr(Round(1 / Cos(1.0471975511966), 6))
End Sub
