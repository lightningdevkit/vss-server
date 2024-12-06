#  Installation guide without docker

## Ubuntu

### Install Java

```sh
sudo apt update
sudo apt install openjdk-17-jdk
java -version
```

### Install and configure PostgreSQL

```sh
sudo apt install postgresql

sudo -u postgres psql
ALTER USER postgres with encrypted password '<password>';
quit
sudo systemctl restart postgresql.service
```

#### Verify
```sh
psql postgresql://postgres:<password>@localhost:5432/postgres`
quit
```

### Install Gradle

```
sudo apt intall unzip
wget https://services.gradle.org/distributions/gradle-8.11.1-bin.zip  -P /tmp
sudo mkdir /opt/gradle
sudo unzip -d /opt/gradle /tmp/gradle-8.11.1-bin.zip
export PATH=$PATH:/opt/gradle/gradle-8.11.1/bin
```

### Install Tomcat

```
sudo useradd -m -U -d /opt/tomcat -s /bin/false tomcat
VERSION=10.1.4
wget https://www-eu.apache.org/dist/tomcat/tomcat-10/v${VERSION}/bin/apache-tomcat-${VERSION}.tar.gz -P /tmp
sudo tar -xf /tmp/apache-tomcat-${VERSION}.tar.gz -C /opt/tomcat/
sudo ln -s /opt/tomcat/apache-tomcat-${VERSION} /opt/tomcat/latest
sudo chown -R tomcat: /opt/tomcat
sudo sh -c 'chmod +x /opt/tomcat/latest/bin/*.sh'
```

#### Systemd for tomcat
location:
```
/etc/systemd/system/tomcat.service
```
content:

```
[Unit]
Description=Tomcat 10 servlet container
After=network.target

[Service]
Type=forking

User=tomcat
Group=tomcat

Environment="JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64"
Environment="JAVA_OPTS=-Djava.security.egd=file:///dev/urandom -Djava.awt.headless=true"

Environment="CATALINA_BASE=/opt/tomcat/latest"
Environment="CATALINA_HOME=/opt/tomcat/latest"
Environment="CATALINA_PID=/opt/tomcat/latest/temp/tomcat.pid"
Environment="CATALINA_OPTS=-Xms512M -Xmx1024M -server -XX:+UseParallelGC"

ExecStart=/opt/tomcat/latest/bin/startup.sh
ExecStop=/opt/tomcat/latest/bin/shutdown.sh

[Install]
WantedBy=multi-user.target
```

#### Enable and start tomcat

```
sudo systemctl daemon-reload
sudo systemctl enable --now tomcat
sudo systemctl status tomcat
```

### Clone repo
```
git clone https://github.com/lightningdevkit/vss-server.git
cd vss-server/java
```

### Configure by editing password for postgres in this file for the same one used in step 2
```
./app/src/main/resources/application.properties
```

### Build
```
gradle wrapper --gradle-version 8.1.1
./gradlew build -x test  # Running tests requires docker-engine to be running.
```

### Deploy
```
sudo cp app/build/libs/vss-1.0.war /opt/tomcat/latest/webapps/vss.war
```

### Verify deployment

```
curl --data-binary "$(echo "0A0773746F726549641A150A026B3110FFFFFFFFFFFFFFFFFF011A046B317631" | xxd -r -p)" http://localhost:8080/vss/putObjects

curl --data-binary "$(echo "0A0773746F7265496412026B31" | xxd -r -p)" http://localhost:8080/vss/getObject
```

### Enable vss in nginx by adding:
```
location /vss/ {
        proxy_pass http://localhost:8080/vss/;
}
```

Restart nginx and try previous step from the external host machine
