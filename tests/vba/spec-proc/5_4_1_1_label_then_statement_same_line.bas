' INV: vbal.5.4.1.1
' SOURCE: spec ms-vbal 5.4.1 identifier followed by colon at line start is a label; the statement after it runs
' EXPECT: after label
Public Sub Main()
Start: Debug.Print "after label"
End Sub
