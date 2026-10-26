' INV: vbal.5.2.4.3
' SOURCE: spec ms-vbal 5.2.4.3 an Event without Public has the same meaning as with Public
' EXPECT: got 4
Public Sub Main()
    Dim l As New Listener
    l.Run
End Sub
'#MODULE Src class
Event Fired(ByVal n As Long)
Public Sub Fire(ByVal n As Long)
    RaiseEvent Fired(n)
End Sub
'#MODULE Listener class
Private WithEvents s As Src
Private Sub s_Fired(ByVal n As Long)
    Debug.Print "got " & CStr(n)
End Sub
Public Sub Run()
    Set s = New Src
    s.Fire 4
End Sub
