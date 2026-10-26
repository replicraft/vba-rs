' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, am does not change an hour other than 12
' EXPECT: 1:5:0
' EXPECT: 11:59:0
Public Sub Main()
    Debug.Print T(#1:05 am#)
    Debug.Print T(#11:59 AM#)
End Sub

Private Function T(ByVal d As Date) As String
    T = Hour(d) & ":" & Minute(d) & ":" & Second(d)
End Function
