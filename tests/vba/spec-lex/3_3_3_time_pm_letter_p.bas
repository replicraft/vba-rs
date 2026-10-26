' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, ampm may be the single letters p and a
' EXPECT: 15:0:0
' EXPECT: 9:0:0
' EXPECT: 16:15:0
Public Sub Main()
    Debug.Print T(#3 p#)
    Debug.Print T(#9 a#)
    Debug.Print T(#4:15 P#)
End Sub

Private Function T(ByVal d As Date) As String
    T = Hour(d) & ":" & Minute(d) & ":" & Second(d)
End Function
