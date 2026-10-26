' INV: vbal.6.1.2.4.1.1, func.dateadd
' SOURCE: spec ms-vbal 6.1.2.4.1.1 DateAdd invalid interval and year-before-100 raise error 5
' EXPECT: x=5
' EXPECT: empty=5
' EXPECT: yyy=5
' EXPECT: mm=5
' EXPECT: before100=5
' EXPECT: year100=0
Public Sub Main()
    Dim v As Variant
    Dim b As Date
    b = DateSerial(1950, 1, 1)
    On Error Resume Next
    Err.Clear: v = DateAdd("x", 1, b): Debug.Print "x=" & Err.Number
    Err.Clear: v = DateAdd("", 1, b): Debug.Print "empty=" & Err.Number
    Err.Clear: v = DateAdd("yyy", 1, b): Debug.Print "yyy=" & Err.Number
    Err.Clear: v = DateAdd("mm", 1, b): Debug.Print "mm=" & Err.Number
    Err.Clear: v = DateAdd("yyyy", -1900, b): Debug.Print "before100=" & Err.Number
    Err.Clear: v = DateAdd("yyyy", -1850, b): Debug.Print "year100=" & Err.Number
End Sub
