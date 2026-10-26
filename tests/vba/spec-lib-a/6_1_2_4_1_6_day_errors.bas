' INV: vbal.6.1.2.4.1.6, vbal.6.1.2.4.1.9, vbal.6.1.2.4.1.14, func.day
' SOURCE: spec ms-vbal 6.1.2.4.1.6 Day/Month/Year argument is Let-coerced to Date (Error and non-date text fail with 13)
' XFAIL: wrong not implemented: error 13 for Day/Year of an Error value
' EXPECT: dayText=13
' EXPECT: monthText=13
' EXPECT: yearText=13
' EXPECT: dayError=13
' EXPECT: yearError=13
Public Sub Main()
    Dim v As Variant
    On Error Resume Next
    Err.Clear: v = Day("not a date"): Debug.Print "dayText=" & Err.Number
    Err.Clear: v = Month("not a date"): Debug.Print "monthText=" & Err.Number
    Err.Clear: v = Year("not a date"): Debug.Print "yearText=" & Err.Number
    Err.Clear: v = Day(CVErr(5)): Debug.Print "dayError=" & Err.Number
    Err.Clear: v = Year(CVErr(5)): Debug.Print "yearError=" & Err.Number
End Sub
