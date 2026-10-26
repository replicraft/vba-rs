' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.2 no overlap within one def-directive (spec-strict: a warning by default, the program runs)
' EXPECT: ok
DefInt A-C, B
Public Sub Main()
    Debug.Print "ok"
End Sub
