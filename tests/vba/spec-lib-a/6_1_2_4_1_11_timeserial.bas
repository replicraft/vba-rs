' INV: vbal.6.1.2.4.1.11, func.timeserial
' SOURCE: spec ms-vbal 6.1.2.4.1.11 TimeSerial normal and overflowing arguments
' XFAIL: wrong pending
' EXPECT: normal=11:59:59
' EXPECT: min75=1:15:0
' EXPECT: negMin=0:30:0
' EXPECT: sec3661=1:1:1
' EXPECT: secNeg=0:59:59
' EXPECT: noon=0.5
' EXPECT: type=Date
' EXPECT: day=1
' EXPECT: arg40000=6
Private Function G(t As Variant) As String
    G = CStr(Hour(t)) & ":" & CStr(Minute(t)) & ":" & CStr(Second(t))
End Function
Public Sub Main()
    Dim v As Variant
    Debug.Print "normal=" & G(TimeSerial(11, 59, 59))
    Debug.Print "min75=" & G(TimeSerial(0, 75, 0))
    Debug.Print "negMin=" & G(TimeSerial(1, -30, 0))
    Debug.Print "sec3661=" & G(TimeSerial(0, 0, 3661))
    Debug.Print "secNeg=" & G(TimeSerial(1, 0, -1))
    Debug.Print "noon=" & CStr(CDbl(TimeSerial(12, 0, 0)))
    Debug.Print "type=" & TypeName(TimeSerial(1, 2, 3))
    Debug.Print "day=" & CStr(CDbl(TimeSerial(0, 1440, 0)))
    On Error Resume Next
    Err.Clear: v = TimeSerial(0, 40000, 0): Debug.Print "arg40000=" & Err.Number
End Sub
