' INV: vbal.6.1.3.2.1.2
' SOURCE: spec ms-vbal 6.1.3.2.1.2 Raise generates a run-time error with the given number
' EXPECT: before
' ERROR: 1051
Public Sub Main()
    Debug.Print "before"
    Err.Raise 1051, "SomeClass"
    Debug.Print "after"
End Sub
