' INV: vbal.6.1.2.4.1.4, func.dateserial
' SOURCE: spec ms-vbal 6.1.2.4.1.4 DateSerial two-digit years default windowing (0-29 -> 2000s, 30-99 -> 1900s)
' EXPECT: y0=2000
' EXPECT: y29=2029
' EXPECT: y30=1930
' EXPECT: y99=1999
' EXPECT: y50=1950
Public Sub Main()
    Debug.Print "y0=" & CStr(Year(DateSerial(0, 1, 1)))
    Debug.Print "y29=" & CStr(Year(DateSerial(29, 1, 1)))
    Debug.Print "y30=" & CStr(Year(DateSerial(30, 1, 1)))
    Debug.Print "y99=" & CStr(Year(DateSerial(99, 1, 1)))
    Debug.Print "y50=" & CStr(Year(DateSerial(50, 6, 15)))
End Sub
