' INV: func.createobject, vbal.6.1.2.8.1.4
' SOURCE: derived the optional servername argument names a remote machine; the embedding application's policy requires the network capability for it, so with only com-create the call is refused (error 70)
' METHOD: by hand
' GRANT: com-create
' ERROR: 70
Public Sub Main()
    Dim d As Object
    Set d = CreateObject("Scripting.Dictionary", "remote-host")
End Sub
