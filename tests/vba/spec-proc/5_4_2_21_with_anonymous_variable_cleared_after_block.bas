' INV: vbal.5.4.2.21, stmt.with
' SOURCE: spec ms-vbal 5.4.2.21 after the block the anonymous With variable is set to Nothing (releasing the object)
' EXPECT: in
' EXPECT: term
' EXPECT: after
'#MODULE Tracker class
Public V As Long
Private Sub Class_Terminate()
    Debug.Print "term"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    With New Tracker
        .V = 1
        Debug.Print "in"
    End With
    Debug.Print "after"
End Sub
