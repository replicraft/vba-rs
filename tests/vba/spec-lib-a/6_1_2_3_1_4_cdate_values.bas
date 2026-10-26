' INV: vbal.6.1.2.3.1.4, func.cdate
' SOURCE: spec ms-vbal 6.1.2.3.1.4 CDate numeric coercion, epoch 12/30/1899
' EXPECT: zero=1899-12-30
' EXPECT: one=1899-12-31
' EXPECT: neg1=1899-12-29
' EXPECT: true=1899-12-29
' EXPECT: half=12:0
' EXPECT: dbl=1.5
' EXPECT: max=9999-12-31
' EXPECT: min=100-1-1
' EXPECT: type=Date
' EXPECT: empty=1899-12-30
Private Function F(d As Variant) As String
    F = CStr(Year(d)) & "-" & CStr(Month(d)) & "-" & CStr(Day(d))
End Function
Public Sub Main()
    Dim e As Variant
    Debug.Print "zero=" & F(CDate(0))
    Debug.Print "one=" & F(CDate(1))
    Debug.Print "neg1=" & F(CDate(-1))
    Debug.Print "true=" & F(CDate(True))
    Debug.Print "half=" & CStr(Hour(CDate(0.5))) & ":" & CStr(Minute(CDate(0.5)))
    Debug.Print "dbl=" & CStr(CDbl(CDate(1.5)))
    Debug.Print "max=" & F(CDate(2958465))
    Debug.Print "min=" & F(CDate(-657434))
    Debug.Print "type=" & TypeName(CDate(1))
    Debug.Print "empty=" & F(CDate(e))
End Sub
