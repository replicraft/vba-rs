' INV: vbal.5.3.1
' SOURCE: spec ms-vbal 5.3.1 end-label must differ from labels in the body
' ERROR: load
Public Sub Main()
    GoTo Done
Done:
    Debug.Print "x"
Done:
End Sub
