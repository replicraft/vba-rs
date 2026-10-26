' INV: vbal.6.1.2.4.1.2, func.datediff
' SOURCE: spec ms-vbal 6.1.2.4.1.2 DateDiff interval case-insensitivity and invalid interval error 5
' EXPECT: D=10
' EXPECT: YYYY=1
' EXPECT: Q=1
' EXPECT: x=5
' EXPECT: empty=5
' EXPECT: yyy=5
Public Sub Main()
    Dim v As Variant
    Dim a As Date
    Dim b As Date
    a = DateSerial(2020, 3, 4)
    b = DateSerial(2020, 3, 14)
    Debug.Print "D=" & CStr(DateDiff("D", a, b))
    Debug.Print "YYYY=" & CStr(DateDiff("YYYY", DateSerial(2019, 12, 31), DateSerial(2020, 1, 1)))
    Debug.Print "Q=" & CStr(DateDiff("Q", DateSerial(2019, 12, 31), DateSerial(2020, 1, 1)))
    On Error Resume Next
    Err.Clear: v = DateDiff("x", a, b): Debug.Print "x=" & Err.Number
    Err.Clear: v = DateDiff("", a, b): Debug.Print "empty=" & Err.Number
    Err.Clear: v = DateDiff("yyy", a, b): Debug.Print "yyy=" & Err.Number
End Sub
