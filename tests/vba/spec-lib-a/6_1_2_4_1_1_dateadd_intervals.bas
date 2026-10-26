' INV: vbal.6.1.2.4.1.1, func.dateadd
' SOURCE: spec ms-vbal 6.1.2.4.1.1 DateAdd interval table
' EXPECT: yyyy=2022-3-4
' EXPECT: q=2020-9-4
' EXPECT: m=2020-5-4
' EXPECT: y=2020-3-6
' EXPECT: d=2020-3-6
' EXPECT: ww=2020-3-18
' EXPECT: neg=2019-3-4
' EXPECT: h=2020-3-5 1:0:0
' EXPECT: n=2020-3-4 1:30:0
' EXPECT: s=2020-3-4 1:1:1
Private Function F(d As Variant) As String
    F = CStr(Year(d)) & "-" & CStr(Month(d)) & "-" & CStr(Day(d))
End Function
Private Function G(d As Variant) As String
    G = F(d) & " " & CStr(Hour(d)) & ":" & CStr(Minute(d)) & ":" & CStr(Second(d))
End Function
Public Sub Main()
    Dim b As Date
    b = DateSerial(2020, 3, 4)
    Debug.Print "yyyy=" & F(DateAdd("yyyy", 2, b))
    Debug.Print "q=" & F(DateAdd("q", 2, b))
    Debug.Print "m=" & F(DateAdd("m", 2, b))
    Debug.Print "y=" & F(DateAdd("y", 2, b))
    Debug.Print "d=" & F(DateAdd("d", 2, b))
    Debug.Print "ww=" & F(DateAdd("ww", 2, b))
    Debug.Print "neg=" & F(DateAdd("yyyy", -1, b))
    Debug.Print "h=" & G(DateAdd("h", 25, b))
    Debug.Print "n=" & G(DateAdd("n", 90, b))
    Debug.Print "s=" & G(DateAdd("s", 3661, b))
End Sub
