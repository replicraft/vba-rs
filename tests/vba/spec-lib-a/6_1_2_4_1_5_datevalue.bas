' INV: vbal.6.1.2.4.1.5, func.datevalue
' SOURCE: spec ms-vbal 6.1.2.4.1.5 DateValue string forms, time part dropped
' EXPECT: numeric=1991-12-30
' EXPECT: longName=1991-12-30
' EXPECT: abbrev=1991-12-30
' EXPECT: withTime=1991-12-30 0:0:0
' EXPECT: type=Date
Private Function F(d As Variant) As String
    F = CStr(Year(d)) & "-" & CStr(Month(d)) & "-" & CStr(Day(d))
End Function
Public Sub Main()
    Debug.Print "numeric=" & F(DateValue("12/30/1991"))
    Debug.Print "longName=" & F(DateValue("December 30, 1991"))
    Debug.Print "abbrev=" & F(DateValue("Dec 30, 1991"))
    Debug.Print "withTime=" & F(DateValue("12/30/1991 3:45 PM")) & " " & CStr(Hour(DateValue("12/30/1991 3:45 PM"))) & ":" & CStr(Minute(DateValue("12/30/1991 3:45 PM"))) & ":" & CStr(Second(DateValue("12/30/1991 3:45 PM")))
    Debug.Print "type=" & TypeName(DateValue("12/30/1991"))
End Sub
