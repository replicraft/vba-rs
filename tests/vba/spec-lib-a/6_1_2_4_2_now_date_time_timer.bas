' INV: vbal.6.1.2.4.2.2, vbal.6.1.2.4.2.3, vbal.6.1.2.4.2.4, vbal.6.1.2.4.2.5, func.now, func.date, func.time, func.timer
' SOURCE: spec ms-vbal 6.1.2.4.2 Date/Now/Time/Timer result types and ranges (values not asserted)
' EXPECT: dateType=Date
' EXPECT: dateDollarType=String
' EXPECT: nowType=Date
' EXPECT: timeType=Date
' EXPECT: timeDollarType=String
' EXPECT: timerType=Single
' EXPECT: dateNoTime=True
' EXPECT: timeNoDate=True
' EXPECT: nowAfterDate=True
' EXPECT: timerRange=True
' EXPECT: yearRange=True
Public Sub Main()
    Dim d As Variant
    Dim t As Variant
    Dim n As Variant
    d = Date
    t = Time
    n = Now
    Debug.Print "dateType=" & TypeName(d)
    Debug.Print "dateDollarType=" & TypeName(Date$)
    Debug.Print "nowType=" & TypeName(n)
    Debug.Print "timeType=" & TypeName(t)
    Debug.Print "timeDollarType=" & TypeName(Time$)
    Debug.Print "timerType=" & TypeName(Timer)
    Debug.Print "dateNoTime=" & CStr(CDbl(d) = Int(CDbl(d)))
    Debug.Print "timeNoDate=" & CStr(CDbl(t) >= 0 And CDbl(t) < 1)
    Debug.Print "nowAfterDate=" & CStr(CDbl(n) >= CDbl(d))
    Debug.Print "timerRange=" & CStr(Timer >= 0 And Timer < 86400)
    Debug.Print "yearRange=" & CStr(Year(n) >= 100 And Year(n) <= 9999)
End Sub
