' INV: vbal.6.1.2.4.1.12, func.timevalue
' SOURCE: spec ms-vbal 6.1.2.4.1.12 TimeValue time string, date part dropped, Null
' EXPECT: pm=15:45:10
' EXPECT: am=0:0:0
' EXPECT: noon=12:0:0
' EXPECT: withDate=15:45:0
' EXPECT: withDateDays=0
' EXPECT: type=Date
' EXPECT: null=True
' EXPECT: dbl=0.5
Private Function G(t As Variant) As String
    G = CStr(Hour(t)) & ":" & CStr(Minute(t)) & ":" & CStr(Second(t))
End Function
Public Sub Main()
    Debug.Print "pm=" & G(TimeValue("3:45:10 PM"))
    Debug.Print "am=" & G(TimeValue("12:00:00 AM"))
    Debug.Print "noon=" & G(TimeValue("12:00:00 PM"))
    Debug.Print "withDate=" & G(TimeValue("12/30/1991 3:45 PM"))
    Debug.Print "withDateDays=" & CStr(Int(CDbl(TimeValue("12/30/1991 3:45 PM"))))
    Debug.Print "type=" & TypeName(TimeValue("1:00 PM"))
    Debug.Print "null=" & CStr(IsNull(TimeValue(Null)))
    Debug.Print "dbl=" & CStr(CDbl(TimeValue("12:00:00 PM")))
End Sub
