' INV: vbal.5.2.3.1.2, decl.withevents
' SOURCE: spec ms-vbal 5.2.3.1.2 variable name plus underscore is the handler name prefix
' EXPECT: handled 3
Public Sub Main()
    Dim l As New Listener
    l.Go
End Sub
'#MODULE Source class
Public Event Ping(ByVal n As Long)
Public Sub Fire(ByVal n As Long)
    RaiseEvent Ping(n)
End Sub
'#MODULE Listener class
Private WithEvents thing As Source
Private Sub thing_Ping(ByVal n As Long)
    Debug.Print "handled " & CStr(n)
End Sub
Public Sub Go()
    Set thing = New Source
    thing.Fire 3
End Sub
