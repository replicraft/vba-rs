' INV: func.timer
' SOURCE: spec ms-vbal 6.1.2.4.2.5 Timer seconds since midnight
' EXPECT: True
Public Sub Main()
    Debug.Print Timer >= 0 And Timer < 86400
End Sub
