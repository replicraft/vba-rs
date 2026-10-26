' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, am or a with hour 12 means hour 0
' XFAIL: wrong pending
' EXPECT: 0:30:0
' EXPECT: 0:0:0
' EXPECT: 0:0:0
Public Sub Main()
    Debug.Print T(#12:30 am#)
    Debug.Print T(#12 am#)
    Debug.Print T(#12 A#)
End Sub

Private Function T(ByVal d As Date) As String
    T = Hour(d) & ":" & Minute(d) & ":" & Second(d)
End Function
