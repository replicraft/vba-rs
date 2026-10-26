' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, pm or p adds 12 to an hour in 0 to 11 (hour 12 unchanged)
' EXPECT: 15:4:5
' EXPECT: 12:30:0
' EXPECT: 23:59:59
' EXPECT: 15:0:0
Public Sub Main()
    Debug.Print T(#3:04:05 pm#)
    Debug.Print T(#12:30 pm#)
    Debug.Print T(#11:59:59 PM#)
    Debug.Print T(#3 PM#)
End Sub

Private Function T(ByVal d As Date) As String
    T = Hour(d) & ":" & Minute(d) & ":" & Second(d)
End Function
