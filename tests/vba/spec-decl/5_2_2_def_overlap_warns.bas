' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.2 no overlap among letter-spec productions (spec-strict: a warning by default, the program runs)
' EXPECT: ok
DefInt A-D
DefLng C-F
Public Sub Main()
    Debug.Print "ok"
End Sub
