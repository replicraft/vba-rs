' INV: vbal.5.2.4.3
' SOURCE: spec ms-vbal 5.2.4.3 event names cannot contain an underscore (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Public Sub Main()
    Debug.Print "ok"
End Sub
'#MODULE Src class
Public Event Did_It()
