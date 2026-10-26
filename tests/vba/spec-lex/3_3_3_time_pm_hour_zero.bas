' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, pm with hour 0 counts as hour 12
' XFAIL: wrong pending
' EXPECT: 12:30:0
Public Sub Main()
    Debug.Print T(#0:30 pm#)
End Sub

Private Function T(ByVal d As Date) As String
    T = Hour(d) & ":" & Minute(d) & ":" & Second(d)
End Function
