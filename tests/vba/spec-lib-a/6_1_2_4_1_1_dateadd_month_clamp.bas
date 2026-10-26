' INV: vbal.6.1.2.4.1.1, func.dateadd
' SOURCE: spec ms-vbal 6.1.2.4.1.1 DateAdd never returns an invalid date (month-end clamping)
' EXPECT: jan31_1995=1995-2-28
' EXPECT: jan31_1996=1996-2-29
' EXPECT: jan31_q=1995-4-30
' EXPECT: feb29_yyyy=2021-2-28
' EXPECT: neg=1995-12-31
' EXPECT: mar31_minus1m=1995-2-28
Private Function F(d As Variant) As String
    F = CStr(Year(d)) & "-" & CStr(Month(d)) & "-" & CStr(Day(d))
End Function
Public Sub Main()
    Debug.Print "jan31_1995=" & F(DateAdd("m", 1, DateSerial(1995, 1, 31)))
    Debug.Print "jan31_1996=" & F(DateAdd("m", 1, DateSerial(1996, 1, 31)))
    Debug.Print "jan31_q=" & F(DateAdd("q", 1, DateSerial(1995, 1, 31)))
    Debug.Print "feb29_yyyy=" & F(DateAdd("yyyy", 1, DateSerial(2020, 2, 29)))
    Debug.Print "neg=" & F(DateAdd("m", -1, DateSerial(1996, 1, 31)))
    Debug.Print "mar31_minus1m=" & F(DateAdd("m", -1, DateSerial(1995, 3, 31)))
End Sub
