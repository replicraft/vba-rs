' INV: vbal.6.1.2.3.1.4, func.cdate
' SOURCE: spec ms-vbal 6.1.2.3.1.4 CDate error cases
' EXPECT: error=13
' EXPECT: above=6
' EXPECT: below=6
' EXPECT: text=13
' EXPECT: null=94
' EXPECT: cvdateError=13
Public Sub Main()
    Dim v As Variant
    On Error Resume Next
    Err.Clear: v = CDate(CVErr(5)): Debug.Print "error=" & Err.Number
    Err.Clear: v = CDate(3000000): Debug.Print "above=" & Err.Number
    Err.Clear: v = CDate(-700000): Debug.Print "below=" & Err.Number
    Err.Clear: v = CDate("not a date"): Debug.Print "text=" & Err.Number
    Err.Clear: v = CDate(Null): Debug.Print "null=" & Err.Number
    Err.Clear: v = CVDate(CVErr(5)): Debug.Print "cvdateError=" & Err.Number
End Sub
