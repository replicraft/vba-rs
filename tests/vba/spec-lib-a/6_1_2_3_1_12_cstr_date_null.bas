' INV: vbal.6.1.2.3.1.12, func.cstr
' SOURCE: spec ms-vbal 6.1.2.3.1.12 CStr of Date (en-US) and Null
' EXPECT: time=12:00:00 PM
' EXPECT: date=3/4/2020
' EXPECT: null=94
Public Sub Main()
    Dim v As Variant
    Debug.Print "time=" & CStr(CDate(0.5))
    Debug.Print "date=" & CStr(DateSerial(2020, 3, 4))
    On Error Resume Next
    Err.Clear: v = CStr(Null): Debug.Print "null=" & Err.Number
End Sub
