' INV: vbal.6.1.2.8.2.2, stmt.beep
' SOURCE: spec ms-vbal 6.1.2.8.2.2 Beep is a Sub with no parameters and no observable result
' EXPECT: after
Public Sub Main()
    Beep
    Debug.Print "after"
End Sub
