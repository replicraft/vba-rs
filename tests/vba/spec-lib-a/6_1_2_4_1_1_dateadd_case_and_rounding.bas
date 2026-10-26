' INV: vbal.6.1.2.4.1.1, func.dateadd
' SOURCE: spec ms-vbal 6.1.2.4.1.1 DateAdd interval is case-insensitive, Number rounded to whole
' EXPECT: YYYY=2021-3-4
' EXPECT: M=2020-4-4
' EXPECT: D=2020-3-5
' EXPECT: Q=2020-6-4
' EXPECT: Yyyy=2021-3-4
' EXPECT: H=1
' EXPECT: N=2
' EXPECT: S=3
' EXPECT: round16=2020-3-6
' EXPECT: round14=2020-3-5
' EXPECT: roundNeg16=2020-3-2
' EXPECT: type=Date
Private Function F(d As Variant) As String
    F = CStr(Year(d)) & "-" & CStr(Month(d)) & "-" & CStr(Day(d))
End Function
Public Sub Main()
    Dim b As Date
    b = DateSerial(2020, 3, 4)
    Debug.Print "YYYY=" & F(DateAdd("YYYY", 1, b))
    Debug.Print "M=" & F(DateAdd("M", 1, b))
    Debug.Print "D=" & F(DateAdd("D", 1, b))
    Debug.Print "Q=" & F(DateAdd("Q", 1, b))
    Debug.Print "Yyyy=" & F(DateAdd("Yyyy", 1, b))
    Debug.Print "H=" & CStr(Hour(DateAdd("H", 1, b)))
    Debug.Print "N=" & CStr(Minute(DateAdd("N", 2, b)))
    Debug.Print "S=" & CStr(Second(DateAdd("S", 3, b)))
    Debug.Print "round16=" & F(DateAdd("d", 1.6, b))
    Debug.Print "round14=" & F(DateAdd("d", 1.4, b))
    Debug.Print "roundNeg16=" & F(DateAdd("d", -1.6, b))
    Debug.Print "type=" & TypeName(DateAdd("d", 1, b))
End Sub
