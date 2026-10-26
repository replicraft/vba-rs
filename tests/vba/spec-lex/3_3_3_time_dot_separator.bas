' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, time-separator may be a dot
' XFAIL: wrong pending
' EXPECT: 3:4:5
' EXPECT: 15:4:0
Public Sub Main()
    Dim d As Date
    d = #3.04.05#
    Debug.Print Hour(d) & ":" & Minute(d) & ":" & Second(d)
    d = #3.04 pm#
    Debug.Print Hour(d) & ":" & Minute(d) & ":" & Second(d)
End Sub
