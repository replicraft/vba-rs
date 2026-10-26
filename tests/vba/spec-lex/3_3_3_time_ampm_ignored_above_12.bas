' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, ampm has no significance when the hour is greater than 12
' EXPECT: 13:30:0
' EXPECT: 13:30:0
' EXPECT: 23:0:0
Public Sub Main()
    Debug.Print T(#13:30 PM#)
    Debug.Print T(#13:30 AM#)
    Debug.Print T(#23:00 pm#)
End Sub

Private Function T(ByVal d As Date) As String
    T = Hour(d) & ":" & Minute(d) & ":" & Second(d)
End Function
