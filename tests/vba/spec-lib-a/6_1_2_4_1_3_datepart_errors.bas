' INV: vbal.6.1.2.4.1.3, func.datepart
' SOURCE: spec ms-vbal 6.1.2.4.1.3 DatePart invalid interval raises error 5
' EXPECT: x=5
' EXPECT: empty=5
' EXPECT: yyy=5
' EXPECT: mm=5
Public Sub Main()
    Dim v As Variant
    Dim t As Date
    t = DateSerial(2020, 3, 4)
    On Error Resume Next
    Err.Clear: v = DatePart("x", t): Debug.Print "x=" & Err.Number
    Err.Clear: v = DatePart("", t): Debug.Print "empty=" & Err.Number
    Err.Clear: v = DatePart("yyy", t): Debug.Print "yyy=" & Err.Number
    Err.Clear: v = DatePart("mm", t): Debug.Print "mm=" & Err.Number
End Sub
