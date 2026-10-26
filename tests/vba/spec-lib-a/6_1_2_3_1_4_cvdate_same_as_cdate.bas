' INV: vbal.6.1.2.3.1.4
' SOURCE: spec ms-vbal 6.1.2.3.1.4 CVDate identical to CDate
' EXPECT: dbl=1.5
' EXPECT: type=Date
' EXPECT: neg1=1899-12-29
' EXPECT: str=1991-12-30
Private Function F(d As Variant) As String
    F = CStr(Year(d)) & "-" & CStr(Month(d)) & "-" & CStr(Day(d))
End Function
Public Sub Main()
    Debug.Print "dbl=" & CStr(CDbl(CVDate(1.5)))
    Debug.Print "type=" & TypeName(CVDate(1))
    Debug.Print "neg1=" & F(CVDate(-1))
    Debug.Print "str=" & F(CVDate("December 30, 1991"))
End Sub
